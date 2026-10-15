# ffx-decomp

Rust reimplementation of Final Fantasy X (Steam HD Remaster), recovered from
`FFX.exe` with REA + Ghidra. Workflow details: `docs/workflow.md`.

## Rules

- Never commit game binaries, `.vbf` archives, extracted files or the
  decompilation dump. Tests find the game through `FFX_GAME_DIR` and skip
  when it is unset.
- Fahrenheit and VBFExtract are references for facts only. Do not copy or
  port their code; derive implementations from FFX.exe and the data.
- Every recovered name goes into `symbols/ffx.sym` (sorted by address, with
  the evidence in the comment). REA sessions lose names on close; reapply
  them with `tools/sym2json.py` → `set_addresses_names`.
- Format docs in `docs/formats/` cite the FFX.exe functions they come from.
- A function replacement needs a parity test in `ffx-oracle` against the
  original machine code before it goes into the hook table
  (`docs/hooking.md`).

## Environment

- Game: `~/steam-games/FFX-X2-HD` (`FFX_GAME_DIR`). Target hash in
  `symbols/ffx.sym`.
- Ghidra: `GHIDRA_INSTALL_DIR=~/tools/ghidra_12.1.4_PUBLIC`. Non-interactive
  shells may not have it; export it before running `rea` or `tools/ghidra/`.
- Decompilation dump: `~/ffx-work/decomp` (`tools/ghidra/dump.sh`). Grep it
  before asking REA; use REA for focused questions.
- Windows tools run from WSL with `.exe`. x64dbg is in
  `C:\Users\USER\x64dbg`; frida-server listens on
  `127.0.0.1:27042` when started.

## Commands

```bash
cargo clippy --all-targets      # pedantic lints, keep it clean
cargo fmt
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test --release -p ffx-vbf --test game_data -- --ignored
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test --release -p ffx-oracle   # parity with the original
tools/build-hook.sh                                                        # 32-bit dinput8.dll
tools/game-tests.sh [--list|--all]                                         # in-game tests for what changed (docs/harness.md)
tools/run-game.sh [--no-hooks] [--lang en]                                 # start FFX.exe on Windows, no Steam needed
tools/run-game.sh --renderer                                               # with the external renderer (docs/renderer.md)
tools/build-renderer.sh                                                    # 64-bit ffx-render.exe
uv run --with frida==17.23.2 tools/frida/peek.py <addr> <len> [--deref]     # read the running game's memory
target/i686-pc-windows-msvc/release/examples/hook-smoke.exe "$(wslpath -w ~/steam-games/FFX-X2-HD/FFX.exe)"
```
