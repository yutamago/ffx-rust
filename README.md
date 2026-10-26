# ffx-decomp

A reimplementation of Final Fantasy X (the Steam HD Remaster) in Rust,
recovered by reverse engineering `FFX.exe`.

You need your own copy of the game. This repository contains no game code or
data, and never will: binaries, archives and extracted files stay out of git.

The goal is all of the game code in Rust: no `FFX.exe`, 64-bit, other
platforms, and mods that can change anything. How we get there:
[docs/rewrite-plan.md](docs/rewrite-plan.md).

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
| Desktop app (FFX Studio, `app/`) | Windows launcher and editors in one window (Tauri + Svelte): Play with or without mods, mods on/off, and editors for items, abilities, equipment, treasure, monsters, encounters, dialogue, models and textures; advanced mode adds scripts, game files and every raw field. Logic in `crates/ffx-studio` with a game-data test of every editor |
| `ffx` CLI | `vbf list/cat/extract/pack`, `kernel tables/export/import`, `monster list/export/import`, `battle list/export/import`, `text export/import`, `script list/disasm/natives/asm`, `texture export/import`, `model export/import`, `motion export/import`, `sound list/wav`, `phyre dump` |
| Function replacement | The `dinput8.dll` proxy hooks 13,905 functions with Rust ports, each with a parity test against the original machine code. The game logic is done (M3, 2026-10-24: walking, events, menus and battles run on Rust logic); the platform code the game reaches (PhyreEngine and the remaster's glue) is being ported (M4: 3,998 of 4,201). Status: [docs/rewrite-plan.md](docs/rewrite-plan.md) |
| Mods | `mods/<mod>/<archive path>` replaces archive files in the running game, also files that grew (the PS2 file layer's sizes are hooked) |
| In-game tests (`ffx-harness`) | The game runs in the background under a TCP harness (buttons, keys, memory, screenshots, file events); tests play into the first scene with an edited event script (grown and relocated; its write read back from memory), a resized model and edited motions, start a battle to check monster, formation and encounter-table mods, and check a kernel mod in memory; saves and settings are sandboxed, randomness seeded, frames run back to back. Recorded replays (walking, events, menus, battles, the boot) compare the ports' run with the original's: events, screenshots, and for some FMOD's mixed audio; draw-stream gates compare the D3D11 calls of a frame |
| Symbol map (`symbols/ffx.sym`) | About 20,900 names with their evidence, about 6,200 of them from the [Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit) team (used with permission, `tools/fh-import.py`) |

## Layout

```text
crates/ffx-core       zlib and path helpers, the first replacements
crates/ffx-bridge     game memory as ports see it (Mem, Host): raw pointers in the game, the emulator in tests
crates/ffx-oracle     runs original FFX.exe functions in an emulator, for parity tests
crates/ffx-hook       the hook table, the patcher, the shims (src/ported), mods, the test harness
crates/ffx-dinput8    dinput8.dll proxy that loads ffx-hook into the game
crates/ffx-harness    launches and drives the game from tests; replays; game-slot
crates/ffx-launch     starts FFX.exe with bottom-up ASLR off, for repeatable addresses
crates/ffx-auto       trivial functions ported by tools/autoport.py
                      game modules (ports, each with its bridge):
crates/ffx-battle     battle (Ms); also the encounter table and formations
crates/ffx-chara      characters (Ch)
crates/ffx-field      field maps, walk meshes, guide map, map effects
crates/ffx-menu       menus, windows and text drawing (Tk, Gu_)
crates/ffx-script     the script VM and natives; disassembler and assembler
crates/ffx-sg         the main loop, fades, scene changes (Sg, FFXApplication)
crates/ffx-sound      music, sound effects and voice (Snd, the FMOD layer)
crates/ffx-system     archives, the PS2 file and CD layers, saves, language, settings
crates/ffx-ps2        the R5900 and VU0 registers the PC port keeps in globals
crates/ffx-vfx        particle effects (ppp, op*)
crates/ffx-movie      movies
crates/ffx-pcui       the PC user interface (title, config, save list, Esc menu)
crates/ffx-graphic    the remaster renderer over PhyreEngine (graphic*, PhyreScene)
                      PhyreEngine and glue ports (M4):
crates/ffx-prender    the render chain and the render-side glue
crates/ffx-pload      serialization and cluster loading
crates/ffx-panim      animation
crates/ffx-pcore      core: matrices, threads, containers, strings
crates/ffx-pother-a   other engine code (class registration, gameplay bindings)
crates/ffx-pother-b   other engine subsystems (physics, input, lighting, morph)
crates/ffx-glue-other the remaster's glue outside rendering (FMOD queue, Iggy, movie)
                      data formats and tools:
crates/ffx-vbf        VBF archive reader and writer
crates/ffx-kernel     battle kernel tables: reader, writer, TOML export/import
crates/ffx-monster    monster files: stats and rewards
crates/ffx-phyre      PhyreEngine clusters: textures to and from DDS, models to glTF
crates/ffx-motion     PS2 skeletons and motion groups: reader, writer, JSON
crates/ffx-text       in-game text encoding
crates/ffx-dxbc       Direct3D shader bytecode and its reflection
crates/ffx-cli        the `ffx` command-line tool
crates/ffx-studio     the desktop app's logic: game, mods, editors, launcher
                      drawing, and the M4 checks:
crates/ffx-drawdiff   compares two captured D3D11 call streams, draw by draw
crates/ffx-draw       the in-process renderer core (pipeline state, D3D11 backend)
crates/ffx-scene      the field scene: what PhyreEngine draws, and a builder for it
crates/ffx-gfx        the scene stream from the game's D3D11 device
crates/ffx-render     the external 64-bit renderer (wgpu; parked)
crates/ffx-audio-census, ffx-platform, ffx-postfx, ffx-uidraw, ffx-fxdraw
                      censuses of the platform boundary (FMOD, window/input, post-processing, UI and effect draws)
app/                  the desktop app: Svelte UI and Tauri window (tools/build-app.sh)
symbols/ffx.sym       recovered names for FFX.exe, the source of truth
symbols/classes.tsv   each game function's category (logic, glue, platform, ...; tools/classify.py)
docs/formats          file format descriptions, with the evidence behind them
tools/ghidra          headless Ghidra scripts (apply symbols, dump decompiled code)
tools/replays         replay scripts and run-all.sh
tools/build-hook.sh   builds the 32-bit dinput8.dll
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
tools/verify-batch.py                         # what a change can affect: tests, port checks, hook build, replays
tools/replays/run-all.sh                      # replays of the ports against the original's recorded runs
```

The replays, the draw-stream gates and the audio comparison are described
in [docs/harness.md](docs/harness.md) and [docs/drawdiff.md](docs/drawdiff.md);
the porting recipe and its checks in [docs/porting.md](docs/porting.md).

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
